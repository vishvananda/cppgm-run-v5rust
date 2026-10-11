// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct box
{
  int f() const;
};

extern template int box<int>::f() const;

int main()
{
  return 0;
}
