template<class T>
struct holder {
  float re;
  float im;

  template<class Tag, int = 0>
  explicit holder(Tag, _Complex float v) : re(__real__ v), im(__imag__ v) {}

  void set_builtin(_Complex float v) {
    re = __real__ v;
    im = __imag__ v;
  }
};

int main() {
  return sizeof(holder<float>) == 0;
}
