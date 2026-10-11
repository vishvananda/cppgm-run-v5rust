namespace std {
template<class T> class initializer_list {
  const T* begin_;
  unsigned long size_;
  initializer_list(const T*, unsigned long);
};
}

struct vector {
  vector(std::initializer_list<int>);
};

void consume(const vector&);

int main()
{
  consume({1});
}
