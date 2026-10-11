// VALIDATION: compile-pass
// N3485 focus: 14.3.2 [temp.arg.nontype], 14.8.2 [temp.deduct]

typedef unsigned long size_t;

template<class T, size_t N>
void swap(T (&a)[N], T (&b)[N]) noexcept;

int main()
{
  return 0;
}
