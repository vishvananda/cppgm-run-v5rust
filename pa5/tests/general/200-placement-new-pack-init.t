// HHC-075
template<class T> T&& declval();
template<class T> T&& forward(T&&);

template<class T, class... Args, class = decltype(::new(declval<void*>()) T(declval<Args>()...))>
T* f(T* p, Args&&... args) { return ((void)0), ::new((void*)p) T(forward<Args>(args)...); }
