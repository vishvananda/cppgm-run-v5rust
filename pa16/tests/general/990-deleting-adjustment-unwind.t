struct A{virtual ~A()noexcept(false){}};struct B{virtual ~B()noexcept(false){}};struct C:A,B{~C()noexcept(false){}};C c;void g(B*p){delete p;}
