namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

template<class T>
struct lazy_leaf {
  T value;
};

template<class... T>
struct lazy_tuple : lazy_leaf<T>... {};

int main() {
  return typeid(lazy_tuple<int[]>*) == typeid(lazy_tuple<int[]>*) ? 0 : 1;
}
