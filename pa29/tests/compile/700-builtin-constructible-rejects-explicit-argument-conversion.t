// VALIDATION: compile-pass

struct optional
{
  explicit operator bool() const;
};

struct field
{
  explicit field(bool);
};

static_assert(!__is_constructible(field, const optional&), "");

int main() {}
