// N3485 5.3.7, 12.8p7: an unevaluated copy retains its recursive exception
// properties and evaluated defaults without odr-using its body; expect exit 0
struct Part {
  Part();
  Part(const Part&);
  Part(Part&&) noexcept;
};
struct Whole { Part part; };
extern Whole whole;
int next();
struct DefaultedArgument {
  DefaultedArgument(const DefaultedArgument&, int = next()) noexcept;
};
extern DefaultedArgument argument;
int main() {
  if (noexcept(Whole(whole))) return 1;
  if (!noexcept(Whole(static_cast<Whole&&>(whole)))) return 2;
  if (noexcept(DefaultedArgument(argument))) return 3;
  return 0;
}
