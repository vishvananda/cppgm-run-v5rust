struct path {
  path(const char *) {}
};

struct library {
  explicit library(const path &) : selected(1) {}
  explicit library(void *) : selected(2) {}

  int selected;
};

int main()
{
  library value("");
  const void * opaque = "";
  return value.selected == 1 && opaque ? 0 : 1;
}
