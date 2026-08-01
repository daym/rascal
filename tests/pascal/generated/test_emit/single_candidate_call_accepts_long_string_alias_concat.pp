unit u;
{$H+}
interface
type
  TCmdStr = AnsiString;
  TScript = class
    procedure Add(const s : TCmdStr);
  end;
  TLinkRes = class(TScript)
    procedure AddFileName(const s : TCmdStr);
  end;
var DirSep : char;
implementation
procedure TScript.Add(const s : TCmdStr); begin end;
procedure TLinkRes.AddFileName(const s : TCmdStr);
begin
  inherited Add('.' + DirSep + s);
end;
end.
