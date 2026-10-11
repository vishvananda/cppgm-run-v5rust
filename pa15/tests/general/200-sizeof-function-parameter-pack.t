template<class... Args>
int g(Args... args2) {
  return sizeof...(args2);
}

int main() { return g(1, 'a') != 2; }
