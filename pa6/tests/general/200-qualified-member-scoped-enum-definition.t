struct writer {
  enum class state : char;
  state st;
};

enum class writer::state : char {
  need_more = 1,
  done = 2
};

writer::state current();

static_assert(sizeof(writer::state) == 1, "underlying type");

