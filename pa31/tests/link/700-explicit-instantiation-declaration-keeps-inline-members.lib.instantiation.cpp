template <class T>
struct Box {
  int size() const;
  inline int compare(int other) const;
  ~Box() { value = -1; }
  int value;
};
template <class T> int Box<T>::size() const { return value * 2; }
template <class T> inline int Box<T>::compare(int other) const { return value - other; }

template int Box<char>::size() const;
