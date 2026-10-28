struct F{virtual void g()throw(int);};struct BadF:F{void g()noexcept(false);};struct N{virtual void f()noexcept;};struct BadN:N{void f()noexcept(false);};
