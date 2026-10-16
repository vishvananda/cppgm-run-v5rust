// N3485 14.7.2p10: a suppressed specialization keeps the prototype
// present when it was formed across later renamed declarations; expect exit 0
template<class T> int suppressed(T);
extern template int suppressed(int);
template<class U> inline int suppressed(U later) { return later; }
template<class T> int visible(T original);
extern template int visible(long);
template<class U> int visible(U renamed);
int main() { return sizeof(suppressed(1)) == sizeof(int) ? 0 : 1; }
