class YA { public: virtual int f() { return 1; } };
class YB { public: virtual int g() { return 2; } };
class YD : public YA, public YB {};
struct Left : virtual YA {};
struct Right : virtual YA {};
struct Diamond : Left, Right {};
struct RepeatedLeft : YB {};
struct RepeatedRight : YB {};
struct Ambiguous : YA, RepeatedLeft, RepeatedRight {};

int main()
{
  YD ordinary;
  YA* primary = &ordinary;
  if (dynamic_cast<YB*>(primary) != static_cast<YB*>(&ordinary)) return 1;

  Diamond diamond;
  YA* shared = &diamond;
  if (dynamic_cast<Right*>(shared) != static_cast<Right*>(&diamond)) return 2;

  Ambiguous ambiguous;
  YA* source = &ambiguous;
  if (dynamic_cast<YB*>(source)) return 3;
  try { throw ambiguous; }
  catch (const YB&) { return 4; }
  catch (...) {}
  return 0;
}
