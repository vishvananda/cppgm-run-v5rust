// Complete class member searches retain their filter and per-name epoch.
struct Parent { enum Number : int; typedef int word; };
struct Derived : Parent {};
using First = Derived::word;
enum Parent::Number : int { late = 7 };
using Second = Derived::word;
using Number = Parent::Number;
int value = Derived::late;
int qualified = Number::late;
