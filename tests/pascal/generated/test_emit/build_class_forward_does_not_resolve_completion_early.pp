unit u;
interface
type
  TScannerFile = class;
  TPreprocTyp = (pp_ifdef, pp_else);
  TPreprocStack = class
    Typ : TPreprocTyp;
    Next : TPreprocStack;
    Owner : TScannerFile;
  end;
  TDirectiveItem = class end;
  TCompileTimePredicate = function : Boolean;
  TScannerFile = class
    Stack : TPreprocStack;
    procedure IfPreprocStack(atyp : TPreprocTyp; compile_time_predicate : TCompileTimePredicate; item : TDirectiveItem);
  end;
implementation
end.
