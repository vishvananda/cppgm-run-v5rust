template<bool B> struct flag {};
template<class T> inline const bool impl = false;
template<> inline const bool impl<float> = true;
template<class T> struct X : flag<impl<T>> {};
X<unsigned long> x;
