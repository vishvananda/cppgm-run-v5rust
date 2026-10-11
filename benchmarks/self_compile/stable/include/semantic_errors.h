#pragma once

#include <stdexcept>
#include <string>

enum class DiagnosticKind
{
  UserError,
  SubstitutionFailure,
  InternalError,
  Warning,
  Note,
};

struct Diagnostic
{
  Diagnostic() {}
  Diagnostic(DiagnosticKind kind_,
             const std::string & message_,
             const std::string & location_ = std::string(),
             const std::string & phase_ = std::string())
    : kind(kind_),
      message(message_),
      location(location_),
      phase(phase_)
  {}

  DiagnosticKind kind = DiagnosticKind::InternalError;
  std::string message;
  std::string location;
  std::string phase;
};

inline std::string format_diagnostic_message(const Diagnostic & diagnostic)
{
  std::string out = diagnostic.message;
  if(!diagnostic.location.empty()) {
    out += diagnostic.location;
  }
  if(!diagnostic.phase.empty()) {
    out += " [phase " + diagnostic.phase + "]";
  }
  return out;
}

inline Diagnostic append_diagnostic_detail(const Diagnostic & diagnostic,
                                           const std::string & detail)
{
  Diagnostic out = diagnostic;
  out.message += detail;
  return out;
}

struct TemplateSubstitutionFailure : std::logic_error
{
  explicit TemplateSubstitutionFailure(const Diagnostic & diagnostic)
    : std::logic_error(format_diagnostic_message(diagnostic)),
      diagnostic_(diagnostic)
  {}

  explicit TemplateSubstitutionFailure(const std::string & what)
    : TemplateSubstitutionFailure(
          Diagnostic{DiagnosticKind::SubstitutionFailure, what, std::string(), std::string()})
  {}

  const Diagnostic & diagnostic() const
  {
    return diagnostic_;
  }

private:
  Diagnostic diagnostic_;
};

struct SemanticDiagnosticError : std::logic_error
{
  explicit SemanticDiagnosticError(const Diagnostic & diagnostic)
    : std::logic_error(format_diagnostic_message(diagnostic)),
      diagnostic_(diagnostic)
  {}

  const Diagnostic & diagnostic() const
  {
    return diagnostic_;
  }

private:
  Diagnostic diagnostic_;
};

struct SemanticSoftFailure : std::logic_error
{
  using std::logic_error::logic_error;
};

struct ExplicitSpecializationAfterInstantiationError : std::logic_error
{
  using std::logic_error::logic_error;
};

struct NoViableConstructorError : SemanticSoftFailure
{
  using SemanticSoftFailure::SemanticSoftFailure;
};

struct NoViableOverloadError : SemanticSoftFailure
{
  using SemanticSoftFailure::SemanticSoftFailure;
};

struct UnknownFunctionError : SemanticSoftFailure
{
  using SemanticSoftFailure::SemanticSoftFailure;
};

struct NotDataMemberExpressionError : SemanticSoftFailure
{
  using SemanticSoftFailure::SemanticSoftFailure;
};

struct DependentQualifiedTypeMissingTypenameError : std::logic_error
{
  using std::logic_error::logic_error;
};

inline Diagnostic make_diagnostic(DiagnosticKind kind,
                                  const std::string & message,
                                  const std::string & location = std::string(),
                                  const std::string & phase = std::string())
{
  Diagnostic diagnostic;
  diagnostic.kind = kind;
  diagnostic.message = message;
  diagnostic.location = location;
  diagnostic.phase = phase;
  return diagnostic;
}

[[noreturn]] inline void throw_substitution_failure(
    const std::string & message,
    const std::string & location = std::string(),
    const std::string & phase = std::string())
{
  throw TemplateSubstitutionFailure(
      make_diagnostic(DiagnosticKind::SubstitutionFailure, message, location, phase));
}

[[noreturn]] inline void throw_user_error(const std::string & message,
                                          const std::string & location = std::string(),
                                          const std::string & phase = std::string())
{
  throw SemanticDiagnosticError(
      make_diagnostic(DiagnosticKind::UserError, message, location, phase));
}

[[noreturn]] inline void throw_internal_error(const std::string & message,
                                              const std::string & location = std::string(),
                                              const std::string & phase = std::string())
{
  throw SemanticDiagnosticError(
      make_diagnostic(DiagnosticKind::InternalError, message, location, phase));
}
