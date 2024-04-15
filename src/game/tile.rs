use crate::{
    random_component,
    util::arena::{Obj, RandomAccess},
};

pub struct MyDemo {
    counter: u32,
}

random_component!(MyDemo);

pub fn build(app: &mut crate::AppBuilder) {
    app.update.add_systems(system_do_stuff);
}

pub fn system_do_stuff(mut rand: RandomAccess<&mut MyDemo>) {
    rand.provide(|| {
        let mut handle = Obj::new(MyDemo { counter: 4 });
        dbg!(handle);

        handle.counter += 1;
        dbg!(handle.counter);

        Obj::destroy(handle);
    });
}
