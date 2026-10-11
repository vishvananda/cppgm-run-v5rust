namespace ns {
static constexpr float mlf = 0.875f;
}

float use_mlf(float x) {
  return x / ns::mlf;
}

int main() {
  return use_mlf(1.0f) > 1.0f ? 0 : 1;
}
