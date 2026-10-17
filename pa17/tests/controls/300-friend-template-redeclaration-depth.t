// N3485 14.5.4p1, 14.6.1p1: a friend template's namespace declaration
// supplies its own parameter positions, and later instantiations reuse it.
// expect exit 0
template<class T> class Owner {
  int value;
  template<class U, class V> friend struct Reader;
public:
  Owner() : value(7) {}
};

template<class T, class U = T> struct Reader {
  static int get(Owner<T> owner) { return owner.value; }
  static int identity(Reader<T, U>*);
};

template<class T, class U>
int Reader<T, U>::identity(Reader<T, U>*) { return 3; }

Owner<int> first;
Owner<char> second;

int main() {
  Reader<int> reader;
  return reader.get(first) + Reader<char>::get(second)
      + reader.identity(&reader) == 17 ? 0 : 1;
}
