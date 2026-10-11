typedef unsigned long size_t;

template<class T, bool B, typename T::storage_type = 0>
struct bit_iterator {};

template<size_t N, size_t S>
struct bitset_base {
  typedef size_t storage_type;
  typedef bit_iterator<bitset_base, false> iterator;
  int data[N];
  bitset_base();
};

template<size_t N, size_t S>
bitset_base<N, S>::bitset_base() : data{0} {}

template<size_t Size>
struct bitset : bitset_base<Size == 0 ? 0 : (Size - 1) / (sizeof(size_t) * 8) + 1, Size> {
  typedef bitset_base<Size == 0 ? 0 : (Size - 1) / (sizeof(size_t) * 8) + 1, Size> base;
  bitset& operator&=(bitset const& rhs);
};

template<size_t Size>
bitset<Size>& bitset<Size>::operator&=(bitset const& rhs)
{
  return *this;
}

int main()
{
  bitset<64> value;
  value &= value;
  return 0;
}
