struct target {};

struct source
{
  explicit operator target*() const;
};

static_assert(__is_constructible(target*, source), "");
static_assert(!__is_convertible(source, target*), "");

int main() {}
