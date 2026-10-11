// N3485 focus: 14.7.1 [temp.inst]/2,8,10: value and address use demand individual definitions.
int calls;
int initialize(int value) { ++calls; return value; }
template<class T> struct selected { static int first, second, constant, dependent, unused; };
template<class T> int selected<T>::first = initialize(3);
template<class T> int selected<T>::second = initialize(7);
template<class T> int selected<T>::constant = 7;
template<class T> int selected<T>::dependent = selected<T>::constant;
template<class T> int selected<T>::unused = T::missing;
int* pointer = &selected<int>::first;
int read_second() { return selected<int>::second; }
int read_dependent() { return selected<int>::dependent; }
int main() {
  return *pointer != 3 || read_second() != 7 || read_dependent() != 7 || calls != 2;
}
