typedef unsigned long size_t;

template<int Words, size_t Size>
struct partial_current_instantiation_bits;

template<class Bits, bool IsConst, typename Bits::storage_type = 0>
struct partial_current_instantiation_iterator;

template<class Bits, bool IsConst, typename Bits::storage_type>
struct partial_current_instantiation_iterator
{
};

template<size_t Size>
struct partial_current_instantiation_bits<1, Size>
{
  typedef size_t storage_type;

  typedef partial_current_instantiation_iterator<
      partial_current_instantiation_bits, false> iterator;

  partial_current_instantiation_bits();
};

template<size_t Size>
partial_current_instantiation_bits<1, Size>::
partial_current_instantiation_bits()
{
}

int main()
{
  partial_current_instantiation_bits<1, 3> value;
  (void)&value;
  return 0;
}
