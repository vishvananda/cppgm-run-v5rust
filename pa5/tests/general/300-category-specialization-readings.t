template<class T> struct primary_value { int word; };
template<> struct primary_value<int> { typedef int word; };
primary_value<int>::word * namespace_pointer;

template<class T> struct primary_type { typedef int word; };
template<> struct primary_type<int> { int word; };

void syntax() {
  primary_value<int>::word * multiplication;
  primary_value<int>::word declaration;
  primary_type<int>::word * declaration_first;
  return sizeof(primary_type<int>::word);
}

template<class T> void dependent_syntax() {
  primary_type<T>::word * dependent_value;
  typename primary_type<T>::word * dependent_type;
}

template<class T> struct outer {
  typedef int word;
  void syntax() { outer<T>::word * current_instantiation; }
};
