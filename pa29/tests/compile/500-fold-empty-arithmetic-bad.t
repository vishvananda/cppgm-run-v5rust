template<class... T> auto f(T... t)->decltype((t+...)){return (t+...);} int x=f();
