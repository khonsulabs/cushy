use std::process::exit;
use std::thread;
use std::time::Duration;

use cushy::figures::units::Lp;
use cushy::reactive::channel;
use cushy::reactive::value::{Destination, Dynamic, MapEach, Source, Switchable, Validations};
use cushy::widget::MakeWidget;
use cushy::widgets::input::{InputValue, MaskedString};
use cushy::widgets::layers::{Modal, OverlayLayer};
use cushy::widgets::{Expand, ProgressBar, Validated};
use cushy::Run;
use kempt::Map;

#[derive(Default, PartialEq)]
enum AppState {
    #[default]
    NewUser,
    LoggedIn {
        username: String,
    },
}

fn main() -> cushy::Result {
    let app_state = Dynamic::<AppState>::default();
    let tooltips = OverlayLayer::default();
    let modals = Modal::new();

    // This example switches between a new user form and a screen once a user
    // has signed up successfully. The api service is simulated using a
    // background task.
    let api = channel::build().on_receive(fake_service).finish();

    let ui = app_state.switcher({
        let tooltips = tooltips.clone();
        let modals = modals.clone();
        move |current_state, app_state| match current_state {
            AppState::NewUser => signup_form(&tooltips, &modals, app_state, &api).make_widget(),
            AppState::LoggedIn { username } => logged_in(username, app_state).make_widget(),
        }
    });

    ui.and(tooltips).and(modals).into_layers().run()
}

#[derive(Default, PartialEq)]
enum NewUserState {
    #[default]
    FormEntry,
    SigningUp,
    Done,
}

fn signup_form(
    tooltips: &OverlayLayer,
    modals: &Modal,
    app_state: &Dynamic<AppState>,
    api: &channel::Sender<FakeApiRequest>,
) -> impl MakeWidget {
    let form_state = Dynamic::<NewUserState>::default();
    let username = Dynamic::<String>::default();
    let password = Dynamic::<MaskedString>::default();
    let password_confirmation = Dynamic::<MaskedString>::default();
    let validations = Validations::default();

    // A network request can take time, so rather than waiting on the API call
    // once we are ready to submit the form, we delegate the login process to a
    // background task using a channel.
    let api_errors = Dynamic::default();
    let login_handler = channel::build()
        .on_receive({
            let form_state = form_state.clone();
            let app_state = app_state.clone();
            let api = api.clone();
            let api_errors = api_errors.clone();
            move |(username, password)| {
                handle_login(
                    username,
                    password,
                    &api,
                    &app_state,
                    &form_state,
                    &api_errors,
                );
            }
        })
        .finish();

    // When we are processing a signup request, we should display a modal with a
    // spinner so that the user can't edit the form or click the sign in button
    // again.
    let signup_modal = modals.new_handle();
    form_state
        .for_each(move |state| match state {
            NewUserState::FormEntry { .. } | NewUserState::Done => signup_modal.dismiss(),
            NewUserState::SigningUp => {
                signup_modal.present(
                    "Signing-up"
                        .and(ProgressBar::indeterminant().spinner().centered())
                        .into_rows()
                        .pad()
                        .centered()
                        .contain(),
                );
            }
        })
        .persist();

    // We use a helper in this file `validated_field` to combine our validation
    // callback and any error returned from the API for this field.
    let username_field = "Username"
        .and(
            validated_field(SignupField::Username, username
                .to_input()
                .placeholder("Username"), &username, &validations, &api_errors, |username| {
                    if username.is_empty() {
                        Err(String::from(
                            "usernames must contain at least one character",
                        ))
                    } else if username.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
                        Err(String::from("usernames must contain only a-z or 0-9"))
                    } else {
                        Ok(())
                    }
                })
                .hint("* required")
                .tooltip(
                    tooltips,
                    "Your username uniquely identifies your account. It must only contain ascii letters and digits.",
                ),
        )
        .into_rows();

    let password_field = "Password"
        .and(
            validated_field(
                SignupField::Password,
                password.to_input().placeholder("Password"),
                &password,
                &validations,
                &api_errors,
                |password| {
                    if password.len() < 8 {
                        Err(String::from("passwords must be at least 8 characters long"))
                    } else {
                        Ok(())
                    }
                },
            )
            .hint("* required, 8 characters min")
            .tooltip(tooltips, "Passwords are always at least 8 bytes long."),
        )
        .into_rows();

    // The password confirmation validation simply checks that the password and
    // confirm password match.
    let password_confirmation_result =
        (&password, &password_confirmation).map_each(|(password, confirmation)| {
            if password == confirmation {
                Ok(())
            } else {
                Err("Passwords must match")
            }
        });

    let password_confirmation_field = "Confirm Password"
        .and(
            password_confirmation
                .to_input()
                .placeholder("Password")
                .validation(validations.validate_result(password_confirmation_result)),
        )
        .into_rows();

    let buttons = "Cancel"
        .into_button()
        .on_click(|_| {
            eprintln!("Sign Up cancelled");
            exit(0)
        })
        .into_escape()
        .tooltip(tooltips, "This button quits the program")
        .and(Expand::empty_horizontally())
        .and(
            "Sign Up"
                .into_button()
                .on_click(validations.when_valid(move |_| {
                    // The form is valid and the sign up button was clickeed.
                    // Send the request to our login handler background task
                    // after setting the state to show the indeterminant
                    // progress modal.
                    form_state.set(NewUserState::SigningUp);
                    login_handler
                        .send((username.get(), password.get()))
                        .unwrap();
                }))
                .into_default(),
        )
        .into_columns();

    username_field
        .and(password_field)
        .and(password_confirmation_field)
        .and(buttons)
        .into_rows()
        .contain()
        .width(Lp::inches(3)..Lp::inches(6))
        .pad()
        .scroll()
        .centered()
}

/// Returns `widget` that is validated using `validate` and `api_errors`.
fn validated_field<T>(
    field: SignupField,
    widget: impl MakeWidget,
    value: &Dynamic<T>,
    validations: &Validations,
    api_errors: &Dynamic<Map<SignupField, String>>,
    mut validate: impl FnMut(&T) -> Result<(), String> + Send + 'static,
) -> Validated
where
    T: Send + 'static,
{
    // Create a dynamic that contains the error for this field, or None.
    let api_error = api_errors.map_each(move |errors| errors.get(&field).cloned());
    // When the underlying value has been changed, we should invalidate the API
    // error since the edited value needs to be re-checked by the API.
    value
        .on_change({
            let api_error = api_error.clone();
            move || {
                api_error.set(None);
            }
        })
        .persist();

    // Each time either the value or the api error is updated, we produce a new
    // validation.
    let validation = (value, &api_error).map_each(move |(value, api_error)| {
        validate(value)?;

        if let Some(error) = api_error {
            Err(error.clone())
        } else {
            Ok(())
        }
    });
    // Finally we return the widget with the merged validation.
    widget.validation(validations.validate_result(validation))
}

fn logged_in(username: &str, app_state: &Dynamic<AppState>) -> impl MakeWidget {
    let app_state = app_state.clone();
    format!("Welcome {username}!")
        .and("Log Out".into_button().on_click(move |_| {
            app_state.set(AppState::NewUser);
        }))
        .into_rows()
        .centered()
}

fn handle_login(
    username: String,
    password: MaskedString,
    api: &channel::Sender<FakeApiRequest>,
    app_state: &Dynamic<AppState>,
    form_state: &Dynamic<NewUserState>,
    api_errors: &Dynamic<Map<SignupField, String>>,
) {
    let response = FakeApiRequestKind::SignUp {
        username: username.clone(),
        password,
    }
    .send_to(api);
    match response {
        FakeApiResponse::SignUpSuccess => {
            app_state.set(AppState::LoggedIn { username });
            form_state.set(NewUserState::Done);
        }
        FakeApiResponse::SignUpFailure(errors) => {
            form_state.set(NewUserState::FormEntry);
            api_errors.set(errors);
        }
    }
}

#[derive(Debug)]
enum FakeApiRequestKind {
    SignUp {
        username: String,
        password: MaskedString,
    },
}

impl FakeApiRequestKind {
    fn send_to(self, api: &channel::Sender<FakeApiRequest>) -> FakeApiResponse {
        let (response_sender, response_receiver) = channel::bounded(1);
        api.send(FakeApiRequest {
            kind: self,
            response: response_sender,
        })
        .expect("service running");
        response_receiver.receive().expect("service to respond")
    }
}

#[derive(Debug)]
struct FakeApiRequest {
    kind: FakeApiRequestKind,
    response: channel::Sender<FakeApiResponse>,
}

#[derive(Debug)]
enum FakeApiResponse {
    SignUpFailure(Map<SignupField, String>),
    SignUpSuccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SignupField {
    Username,
    Password,
}

fn fake_service(request: FakeApiRequest) {
    let response = match request.kind {
        FakeApiRequestKind::SignUp { username, password } => {
            // Simulate this api taking a while
            thread::sleep(Duration::from_secs(1));

            let mut errors = Map::new();
            if username == "admin" {
                errors.insert(
                    SignupField::Username,
                    String::from("admin is a reserved username"),
                );
            }
            if *password == "password" {
                errors.insert(
                    SignupField::Password,
                    String::from("'password' is not a strong password"),
                );
            }

            if errors.is_empty() {
                FakeApiResponse::SignUpSuccess
            } else {
                FakeApiResponse::SignUpFailure(errors)
            }
        }
    };
    let _ = request.response.send(response);
}
