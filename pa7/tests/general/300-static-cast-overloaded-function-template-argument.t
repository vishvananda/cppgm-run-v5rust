template<class T> void hello(T);
template<class T> void hello(T, int);
template<class T> void take(T);
struct stream {};
void use() { take(static_cast<void(*)(stream)>(&hello<stream>)); }
