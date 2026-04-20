use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    struct FamilyMember {
        tv: Rc<RefCell<TV>>,
    }

    #[derive(Debug)]
    struct TV {
        channel: String,
    }

    fn member_start_watch_tv() -> FamilyMember {
        let tv_is_on = Rc::new(RefCell::new(TV{channel:"BBC".to_string()}));
        FamilyMember {
            tv: tv_is_on, 
        }
    }

    let dad = member_start_watch_tv();
    let mom = FamilyMember { tv: Rc::clone(&dad.tv) };
    println!("TV channel for mom {:?}", mom.tv);

    let mut remote_control = dad.tv.borrow_mut();
    println!("TV channel {:?}", remote_control);

    remote_control.channel = "MTV".to_string();
    println!("TV channel {:?}", remote_control);
    drop(remote_control);
    println!("TV channel for mom {:?}", mom.tv);
}