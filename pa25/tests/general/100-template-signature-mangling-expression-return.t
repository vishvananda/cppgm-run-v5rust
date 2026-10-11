// Itanium C++ ABI 5.1.5, 5.1.8, N3485 14.5.6.1: a function template
// specialization is mangled with the template's unsubstituted signature, so
// g<int> from enable_if<(sizeof(U) >= 4)> and from enable_if<(sizeof(U) <=
// 4)>, different templates in the two units, get different names
// (..XgestT_Li4EE.. and ..XlestT_Li4EE..)
