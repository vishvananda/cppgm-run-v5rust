namespace lib {
struct token { int x; };
struct carrier : token {};
}

namespace app {
struct token { int y; };
struct derived : lib::carrier, token {};
}

int main() {
  app::derived d;
  app::token *p = &d;
  p->y = 7;
  return p->y != 7;
}
