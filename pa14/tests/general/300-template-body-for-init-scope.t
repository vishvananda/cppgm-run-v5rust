template<class T>
struct loop_body {
  void run() {
    for(int i = 0; i != 1; ++i) {
      (void)i;
    }
  }
};

int main() { return 0; }
