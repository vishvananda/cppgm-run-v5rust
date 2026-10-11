float f(int const & p) {
  return true ? reinterpret_cast<float const &>(p) : 0.0f;
}
int main() {}
