template<class T>struct Child:T{void f()throw(double){}};struct O{struct E;struct B{virtual void f()throw(E){}};Child<B>child;struct E{};};O object;
