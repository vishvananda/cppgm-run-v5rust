int main()
{
  auto invoke = []<class T>(T value)
      __attribute__((__no_sanitize__("memory"))) noexcept {
    return value;
  };
  return invoke(0);
}
