template<class T>
struct SystemBuffer
{
  T value;

  __attribute__((__exclude_from_explicit_instantiation__))
  SystemBuffer(T input) : value(input) {}
  __attribute__((__exclude_from_explicit_instantiation__))
  SystemBuffer(const SystemBuffer& other) : value(other.value + 1) {}
  __attribute__((__exclude_from_explicit_instantiation__))
  SystemBuffer(SystemBuffer&& other) : value(other.value)
  {
    other.value = 0;
  }
};

extern template struct SystemBuffer<int>;
