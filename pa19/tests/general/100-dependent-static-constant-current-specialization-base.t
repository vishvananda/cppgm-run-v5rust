template<unsigned long Words, unsigned long Size>
struct storage {};

template<unsigned long Size>
struct bits {
  static const unsigned words =
      Size == 0 ? 0 : (Size - 1) / (sizeof(unsigned long) * 8) + 1;
  typedef storage<words, Size> base;
  bits& operator&=(const bits&);
};

template<unsigned long Size>
bits<Size>& bits<Size>::operator&=(const bits&) {
  return *this;
}

int main() {
  return 0;
}
