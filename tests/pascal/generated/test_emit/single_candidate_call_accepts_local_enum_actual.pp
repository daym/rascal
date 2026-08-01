unit u;
interface
type
  thost = class
    procedure run;
  end;
implementation
procedure thost.run;
type
  tterminationkind = (term_none, term_string);
procedure newstatement(kind : tterminationkind);
begin
end;
begin
  newstatement(term_none);
end;
end.
