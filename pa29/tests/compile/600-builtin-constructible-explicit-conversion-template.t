struct target {};

struct source
{
  template<class T> explicit operator T() const;
};

static_assert(__is_constructible(target, source), "");
static_assert(!__is_convertible(source, target), "");

int main() {}
