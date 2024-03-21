use windows::Win32::Graphics::{
    Gdi::{BeginPaint, EndPaint},
    OpenGL::{
        glBegin, glClear, glClearColor, glColor3f, glEnable, glEnd, glFlush, glVertex2i,
        glViewport, GL_COLOR, GL_TRIANGLES,
    },
};

use super::Graphics;

pub struct Renderer {}

impl Renderer {
    pub fn new(gfx: &Graphics) -> Self {
        Self {}
    }

    pub fn render(&self, frame: &Graphics) {
        let [width, height] = frame.window.inner_size();
        let mut painter = Default::default();

        unsafe {
            glClearColor(0., 0., 0., 1.);
            glClear(GL_COLOR);
            glBegin(GL_TRIANGLES);
            glColor3f(1., 0., 0.);
            glVertex2i(0, 0);
            glColor3f(0., 1., 0.);
            glVertex2i(1, 0);
            glColor3f(0., 0., 1.);
            glVertex2i(0, 1);
            glEnd();
            glFlush();

            BeginPaint(**frame.window, &mut painter);
            EndPaint(**frame.window, &painter);
        }
    }
}
