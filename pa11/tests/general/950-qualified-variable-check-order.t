// Variable specifier checks precede resolution of the final qualified member.
namespace N {}
mutable int N::missing;
