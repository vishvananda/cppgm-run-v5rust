// N3485 14.5.6.1p5, Itanium C++ ABI 5.1.5: function templates whose return
// types are decltype of a new-expression or a delete-expression are mangled
// with those expressions, so their specializations and static locals are the
// same entities in every unit, whichever compiler builds it
