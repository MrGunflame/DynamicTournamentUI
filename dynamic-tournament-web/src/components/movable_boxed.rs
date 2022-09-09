use std::cell::UnsafeCell;

use gloo_events::{EventListener, EventListenerOptions};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, MouseEvent, TouchEvent};
use yew::prelude::*;

use crate::{
    components::button::Button,
    utils::{document, Rc},
};

// Zoom factor for mouse/touch scroll events.
const ZOOM_FACTOR: f32 = 0.05;

pub struct MovableBoxed {
    /// NodeRef to the current movable-boxed element.
    element: NodeRef,
    /// The <body> element.
    body: HtmlElement,

    mouse_listeners: Option<[EventListener; 3]>,
    touch_listeners: Option<[EventListener; 3]>,

    translate: Coordinates,
    last_move: Coordinates,
    scale: u32,

    /// Whether the box is currently allowed to be moved.
    is_moving: Rc<UnsafeCell<bool>>,
    is_locked: Rc<UnsafeCell<bool>>,
}

impl Component for MovableBoxed {
    type Message = Message;
    type Properties = Properties;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            element: NodeRef::default(),
            body: document().body().unwrap(),

            mouse_listeners: None,
            touch_listeners: None,

            translate: Coordinates::default(),
            last_move: Coordinates::default(),
            scale: 100,

            is_moving: Rc::new(UnsafeCell::new(false)),
            is_locked: Rc::new(UnsafeCell::new(false)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Message::Reposition => {
                self.translate = Coordinates::default();
                self.scale = 100;
            }
            Message::Move(coords) => {
                self.translate.x = self
                    .translate
                    .x
                    .saturating_sub(self.last_move.x.saturating_sub(coords.x));

                self.translate.y = self
                    .translate
                    .y
                    .saturating_sub(self.last_move.y.saturating_sub(coords.y));

                self.last_move.x = coords.x;
                self.last_move.y = coords.y;
            }
            Message::MouseDown(coords) => {
                self.last_move = coords;

                // SAFETY: Since events are executed synchronously on the main thread
                // no other references exist.
                unsafe {
                    let is_moving = &mut *self.is_moving.get();
                    *is_moving = true;
                }
            }
            Message::MouseUp => {
                // SAFETY: Since events are executed synchronously on the main thread
                // no other references exist.
                unsafe {
                    let is_moving = &mut *self.is_moving.get();
                    *is_moving = false;
                }
            }
            Message::ZoomIn(amount) => self.scale = self.scale.saturating_add(amount),
            Message::ZoomOut(amount) => self.scale = self.scale.saturating_sub(amount),
            Message::ToggleLock => unsafe {
                let is_locked = &mut *self.is_locked.get();
                *is_locked = !*is_locked;
            },
        }

        true
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if !first_render {
            return;
        }

        let options = EventListenerOptions::enable_prevent_default();

        let is_moving = self.is_moving.clone();
        let is_locked = self.is_locked.clone();

        let onmousedown = ctx.link().batch_callback(move |event: MouseEvent| {
            if unsafe { *is_locked.get() } {
                return None;
            }

            event.prevent_default();

            Some(Message::MouseDown(Coordinates {
                x: event.client_x(),
                y: event.client_y(),
            }))
        });

        let onmousemove = ctx.link().batch_callback(move |event: MouseEvent| {
            let is_moving = unsafe { *is_moving.get() };

            if is_moving {
                event.prevent_default();

                Some(Message::Move(Coordinates {
                    x: event.client_x(),
                    y: event.client_y(),
                }))
            } else {
                None
            }
        });

        let onmouseup = ctx.link().callback(move |_: MouseEvent| Message::MouseUp);

        let element = self.element.cast::<HtmlElement>().unwrap();
        let mousedown =
            EventListener::new_with_options(&element, "mousedown", options, move |event| {
                onmousedown.emit(event.dyn_ref::<MouseEvent>().unwrap().clone());
            });

        let mousemove =
            EventListener::new_with_options(&self.body, "mousemove", options, move |event| {
                onmousemove.emit(event.dyn_ref::<MouseEvent>().unwrap().clone());
            });

        let mouseup = EventListener::new(&self.body, "mouseup", move |event| {
            onmouseup.emit(event.dyn_ref::<MouseEvent>().unwrap().clone());
        });

        self.mouse_listeners = Some([mousedown, mousemove, mouseup]);

        if let Some(element) = self.element.cast::<HtmlElement>() {
            let is_locked = self.is_locked.clone();
            let ontouchstart = ctx.link().batch_callback(move |event: TouchEvent| {
                if unsafe { *is_locked.get() } {
                    return None;
                }

                let touch = event.touches().get(0).unwrap();

                Some(Message::MouseDown(Coordinates {
                    x: touch.client_x(),
                    y: touch.client_y(),
                }))
            });

            let is_locked = self.is_locked.clone();
            let ontouchmove = ctx.link().batch_callback(move |event: TouchEvent| {
                if unsafe { *is_locked.get() } {
                    return None;
                }

                event.prevent_default();

                let touch = event.touches().get(0).unwrap();

                Some(Message::Move(Coordinates {
                    x: touch.client_x(),
                    y: touch.client_y(),
                }))
            });

            let ontouchend = ctx.link().callback(move |_: ()| Message::MouseUp);

            let touchstart =
                EventListener::new_with_options(&element, "touchstart", options, move |event| {
                    ontouchstart.emit(event.dyn_ref::<TouchEvent>().unwrap().clone());
                });

            let touchmove =
                EventListener::new_with_options(&element, "touchmove", options, move |event| {
                    ontouchmove.emit(event.dyn_ref::<TouchEvent>().unwrap().clone());
                });

            let touchend = EventListener::new(&element, "touchend", move |_| {
                ontouchend.emit(());
            });

            self.touch_listeners = Some([touchstart, touchmove, touchend]);
        }
    }

    // Reposition when props change.
    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        self.update(ctx, Message::Reposition)
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let is_locked = self.is_locked.clone();

        let on_wheel = ctx.link().batch_callback(move |e: WheelEvent| {
            if unsafe { *is_locked.get() } {
                return None;
            }

            e.prevent_default();
            if e.delta_y().is_sign_positive() {
                Some(Message::ZoomOut((e.delta_y() as f32 * ZOOM_FACTOR) as u32))
            } else {
                Some(Message::ZoomIn((-e.delta_y() as f32 * ZOOM_FACTOR) as u32))
            }
        });

        let on_reposition = ctx.link().callback(|_| Message::Reposition);

        let on_zoom_in = ctx.link().callback(|_| Message::ZoomIn(5));

        let on_zoom_out = ctx.link().callback(|_| Message::ZoomOut(5));

        let on_lock = ctx.link().callback(|_| Message::ToggleLock);

        let cursor = if unsafe { *self.is_locked.get() } {
            "cursor: unset;"
        } else if unsafe { *self.is_moving.get() } {
            "cursor: grabbing;"
        } else {
            "cursor: grab;"
        };

        let style = format!(
            "transform: translate({}px, {}px) scale({}%);",
            self.translate.x, self.translate.y, self.scale
        );

        let lock_button = if unsafe { *self.is_locked.get() } {
            html! {
                <button class="button" onclick={on_lock} title="Unlock">
                    <i aria-hidden="true" class="fa-solid fa-lock-open"></i>
                    <span class="sr-only">{ "Unlock" }</span>
                </button>
            }
        } else {
            html! {
                <button class="button" onclick={on_lock} title="Lock">
                    <i aria-hidden="true" class="fa-solid fa-lock"></i>
                    <span class="sr-only">{ "Lock" }</span>
                </button>
            }
        };

        let classes = match ctx.props().classes {
            Some(classes) => format!("movable-boxed {}", classes),
            None => "movable-boxed".to_owned(),
        };

        html! {
            <div ref={self.element.clone()} class={classes} onwheel={on_wheel} style={cursor}>
                <div class="movable-boxed-buttons">
                    <Button onclick={on_reposition} title="Reposition">
                        <i aria-hidden="true" class="fa-solid fa-compress"></i>
                        <span class="sr-only">{ "Reposition" }</span>
                    </Button>
                    <button class="button" onclick={on_zoom_in} title="Zoom In">
                        <i aria-hidden="true" class="fa-solid fa-plus"></i>
                        <span class="sr-only">{ "Zoom In" }</span>
                    </button>
                    <button class="button" onclick={on_zoom_out} title="Zoom Out">
                        <i aria-hidden="true" class="fa-solid fa-minus"></i>
                        <span class="sr-only">{ "Zoom Out" }</span>
                    </button>
                    {lock_button}
                </div>
                <div class="movable-boxed-content" style={style}>
                    { for ctx.props().children.iter() }
                </div>
            </div>
        }
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct Properties {
    pub children: Children,
    pub classes: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Reposition,
    Move(Coordinates),
    MouseUp,
    MouseDown(Coordinates),
    ZoomIn(u32),
    ZoomOut(u32),
    ToggleLock,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Coordinates {
    x: i32,
    y: i32,
}
