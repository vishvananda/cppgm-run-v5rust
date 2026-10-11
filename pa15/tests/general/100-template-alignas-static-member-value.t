template<class T>
struct alignment_of
{
  static const unsigned int value = sizeof(T);
};

template<class T>
struct alignas(alignment_of<T>::value) aligned_value
{
  char data[16];
};

aligned_value<int> global_value;

int main()
{
  return alignof(aligned_value<int>) == sizeof(int) ? 0 : 1;
}
