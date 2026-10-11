// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<class T>
struct box {};

extern template class box<int>;

int main()
{
  return 0;
}
