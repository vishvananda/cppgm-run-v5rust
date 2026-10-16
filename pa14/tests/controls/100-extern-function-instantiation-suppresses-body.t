// N3485 14.7.2p10: end-of-unit suppression of an unused body's dependency; expect exit 0
template<class T> int hidden(T value) { return T::missing; }
inline int unused() { return hidden(0); }
extern template int hidden(int);
extern template int hidden(int);

template<class T> inline int read(T value) { return value + 3; }
extern template int read(int);

int main() { return read(2) == 5 ? 0 : 1; }
