// A base initializer `Base()` value-initializes a base whose default
// constructor is not user-provided -- implicit, or defaulted on its first
// declaration -- zeroing it before any constructor runs.
inline void* operator new(unsigned long, void* place) noexcept { return place; }
struct Defaulted { Defaulted() = default; int x; char c; };
struct Implicit { int x; int y; };
struct Builds : Defaulted { Builds() : Defaulted(), y(7) {} char y; };
struct Aggregates : Implicit { Aggregates() : Implicit(), w(8) {} int w; };

alignas(16) unsigned char buffer[32];

void fill()
{
  for (unsigned i = 0; i < sizeof buffer; ++i) buffer[i] = 0x55;
}

int main()
{
  fill();
  Builds* built = new (buffer) Builds;
  if (built->x != 0 || built->c != 0 || built->y != 7) return 1;
  fill();
  Aggregates* aggregate = new (buffer) Aggregates;
  if (aggregate->x != 0 || aggregate->y != 0 || aggregate->w != 8) return 2;
  return 0;
}
