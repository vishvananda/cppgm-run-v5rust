template<class Pred>
int invoke(Pred pred) {
  return pred(' ') ? 0 : 1;
}

int main() {
  return invoke([](unsigned char ch) { return ch == ' '; });
}
