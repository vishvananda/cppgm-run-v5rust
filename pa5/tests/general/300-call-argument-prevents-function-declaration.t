struct tag {};
int next();
struct holder { holder(tag, int); };
void build() {
  holder h(tag(), next());
}
