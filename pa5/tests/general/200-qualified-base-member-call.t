namespace std {
template<class T, class U>
struct pair {
  void swap(pair&);
};
}

template<class I>
struct sub_match : std::pair<I, I> {
  void swap(sub_match& s) {
    this->std::pair<I, I>::swap(s);
  }
};
