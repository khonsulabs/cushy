use std::collections::HashMap;

use cushy::value::{Dynamic, MapEach};
use cushy::widget::{Children, MakeWidget};
use cushy::widgets::input::InputValue;
use cushy::Run;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Contact {
    pub id: u64,
    pub first_name: String,
    pub last_name: String,
    pub title: String,
}

fn main() -> cushy::Result {
    let initial_contacts = vec![
        Contact {
            id: 0,
            first_name: String::from("John"),
            last_name: String::from("Doe"),
            title: String::from("Chef"),
        },
        Contact {
            id: 1,
            first_name: String::from("Jane"),
            last_name: String::from("Smith"),
            title: String::from("Doctor"),
        },
    ];
    let db = Dynamic::new(
        initial_contacts
            .into_iter()
            .map(|contact| (contact.id, contact))
            .collect::<HashMap<_, _>>(),
    );
    let selected_contact = Dynamic::new(None::<u64>);
    let contact_list = db.map_each({
        let selected_contact = selected_contact.clone();
        move |contacts| {
            let mut entries = contacts
                .iter()
                .map(|(id, contact)| (contact.last_name.clone(), contact.first_name.clone(), *id))
                .collect::<Vec<_>>();
            entries.sort();
            entries
                .into_iter()
                .map(|(last, first, id)| {
                    selected_contact
                        .new_select(Some(id), format!("{first} {last}").align_left())
                        .make_widget()
                })
                .collect::<Children>()
        }
    });

    let editing_contact = (&selected_contact, &db).map_each({
        let db = db.clone();
        move |(selected, contacts)| {
            selected
                .map(|id| edit_contact_form(&contacts[&id], &db).make_widget())
                .unwrap_or_else(|| "Select a contact".centered().make_widget())
        }
    });
    contact_list
        .into_rows()
        .vertical_scroll()
        .and(editing_contact.expand())
        .into_columns()
        .run()
}

fn edit_contact_form(contact: &Contact, db: &Dynamic<HashMap<u64, Contact>>) -> impl MakeWidget {
    let first = Dynamic::new(contact.first_name.clone());
    let last = Dynamic::new(contact.last_name.clone());
    let title = Dynamic::new(contact.title.clone());

    "First Name"
        .and(first.clone().into_input())
        .and("Last Name")
        .and(last.clone().into_input())
        .and("Title")
        .and(title.clone().into_input())
        .and(
            "Save"
                .into_button()
                .on_click({
                    let contact_id = contact.id;
                    let db = db.clone();
                    move |()| {
                        let mut db = db.lock();
                        let contact = db.get_mut(&contact_id).expect("missing contact");
                        contact.first_name = first.get();
                        contact.last_name = last.get();
                        contact.title = title.get();
                    }
                })
                .into_default()
                .align_right(),
        )
        .into_rows()
}
