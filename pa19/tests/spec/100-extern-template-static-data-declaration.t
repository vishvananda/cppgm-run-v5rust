// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct box
{
  static const int n;
};

extern template const int box<int>::n;

int main()
{
  return 0;
}
