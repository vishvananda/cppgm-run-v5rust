void*operator new[](unsigned long,void*) noexcept;int n();void q(void*p){static_assert(noexcept(new(p) int[n()]),"");}
