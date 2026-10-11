void cleanup();
void may_throw();

struct Guard {
  ~Guard() {
    cleanup();
  }
};

int first() {
  Guard guard;
  may_throw();
  return 1;
}

int second() {
  Guard guard;
  may_throw();
  return 2;
}

int main() {
  first();
  return second() == 2 ? 0 : 1;
}
