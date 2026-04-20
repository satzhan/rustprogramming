use std::rc::Rc;

struct FamilyMember {
    tv: Rc<TV>,
}

struct TV;

fn main() {
    fn member_start_watch_tv() -> FamilyMember {
        let tv_is_on = Rc::new(TV);
        FamilyMember {
            tv: Rc::clone(&tv_is_on),
        }
    }

    let dad = member_start_watch_tv();
    println!("How many people watching {}", Rc::strong_count(&dad.tv));

    let mom = FamilyMember { tv: Rc::clone(&dad.tv) };
    println!("How many people watching {}", Rc::strong_count(&dad.tv));

    let me = FamilyMember { tv: Rc::clone(&dad.tv) };
    println!("How many people watching {}", Rc::strong_count(&me.tv));

    drop(dad);
    drop(me);

    println!("How many people watching {}", Rc::strong_count(&mom.tv));
}