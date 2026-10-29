// Objects outside the current try belong to its handler, not the condition.
struct Guard { ~Guard(); };
bool first();
bool second();
bool evaluate() {
  Guard outside;
  try {
    if (first() || second()) return true;
  } catch (int) {}
  return false;
}
