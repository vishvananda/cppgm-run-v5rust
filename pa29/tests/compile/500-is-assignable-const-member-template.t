struct ignore {
  template<class T>
  const ignore& operator=(const T&) const { return *this; }
};

struct mutable_only {
  template<class T>
  mutable_only& operator=(const T&) { return *this; }
};

static_assert(__is_assignable(const ignore&, const bool&), "");
static_assert(!__is_assignable(const mutable_only&, const bool&), "");

int main() {}
