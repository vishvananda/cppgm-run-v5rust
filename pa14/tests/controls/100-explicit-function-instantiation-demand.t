// N3485 14.7.2: definitions may follow explicit instantiation; expect exit 0
template<class T> int read(T);
extern template int read(int);
template int read(int);
template<class U> int read(U value) { return value + 3; }

template<class T> int read(T* value) { return *value + 7; }
template int read(int*);

class Private {
  typedef int integer;
};
template<class T> int unused(T value, int other = T::missing) {
  return value + other;
}
template int unused<Private::integer>(int, int);

int main() {
  int value = 2;
  return read(2) == 5 && read(&value) == 9 && unused(1, 2) == 3 ? 0 : 1;
}
