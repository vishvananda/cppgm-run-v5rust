template<class T>struct Query{static_assert(!noexcept(((T*)0)->f()),"");};struct O{struct E;struct B{void f()throw(E){}};Query<B>q;struct E{};};void use(O::B&b){b.f();}
