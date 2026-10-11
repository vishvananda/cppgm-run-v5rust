constexpr int filtered_sum() {
  int i = 0;
  int s = 0;
  while (1) {
    i += 1;
    if (i == 3) {
      continue;
    }
    if (i == 6) {
      break;
    }
    s += i;
  }
  return s;
}

static_assert(filtered_sum() == 12, "while break continue");

int main() { return 0; }
