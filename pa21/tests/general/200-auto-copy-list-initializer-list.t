namespace std { template<typename T> class initializer_list; }

enum class Format {
  general = 1,
  scientific = 2,
  fixed = 4,
  hex = 8
};

int main() {
  auto fmts = {Format::general, Format::scientific, Format::fixed, Format::hex};
  int mask = 0;
  for (auto fmt : fmts) {
    mask = mask + static_cast<int>(fmt);
  }
  return mask == 15 ? 0 : 1;
}
