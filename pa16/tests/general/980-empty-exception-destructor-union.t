template<class...T>struct B{virtual ~B()throw(T...);};struct M{~M()noexcept(false);};struct D:B<>{M m;};D*p;
