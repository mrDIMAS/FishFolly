use fyrox::{
    core::pool::Handle,
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        Thickness, UiNode,
    },
    plugin::PluginContext,
};

pub struct Menu {
    root: Handle<UiNode>,
    exit: Handle<UiNode>,
}

impl Menu {
    pub fn new(context: &mut PluginContext) -> Self {
        let ctx = &mut context.user_interface.build_ctx();

        let exit;
        let root = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false) // TODO
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(1)
                            .with_child(
                                ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Start")
                                .build(ctx),
                            )
                            .with_child({
                                exit = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Exit")
                                .build(ctx);
                                exit
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        Self { root, exit }
    }

    pub fn handle_ui_message(
        &mut self,
        _context: &mut PluginContext,
        message: &UiMessage,
        control_flow: &mut ControlFlow,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.exit {
                *control_flow = ControlFlow::Exit;
            }
        }
    }

    pub fn handle_os_event(&mut self, event: &Event<()>, context: PluginContext) {
        if let Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } = event
        {
            context.user_interface.send_message(WidgetMessage::width(
                self.root,
                MessageDirection::ToWidget,
                size.width as f32,
            ));
            context.user_interface.send_message(WidgetMessage::height(
                self.root,
                MessageDirection::ToWidget,
                size.height as f32,
            ));
        }
    }
}
