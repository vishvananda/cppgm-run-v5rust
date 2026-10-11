struct target {};

struct source
{
  explicit operator target() const;
};

target convert(source value) { return target(value); }

int main() {}
