unit sysutils;
interface

type
  TExecuteFlags = Set of (ExecInheritsHandles);
  Exception = class(TObject)
  private
    FMessage: String;
    FHelpContext: LongInt;
  public
    constructor Create(const Msg: String);
    constructor CreateHelp(const Msg: String; AHelpContext: LongInt);
    property HelpContext: LongInt read FHelpContext write FHelpContext;
    property Message: String read FMessage write FMessage;
  end;
  EIntError = class(Exception);
  ERangeError = class(EIntError);
  EIntOverflow = class(EIntError);
  EHeapMemoryError = class(Exception);
  EOutOfMemory = class(EHeapMemoryError);
  EAccessViolation = class(Exception);
  EAbstractError = class(Exception);
  EInOutError = class(Exception)
  public
    ErrorCode: Integer;
  end;
  EOSError = class(Exception)
  public
    ErrorCode: Integer;
  end;

function Supports(a: TObject; b: TClass): Boolean; external name '::u_system::p_supports';
function CompareText(const S1: AnsiString; const S2: AnsiString): Integer;

implementation

function CompareText(const S1: AnsiString; const S2: AnsiString): Integer;
var
  I, Count, Count1, Count2: SizeInt;
  Chr1, Chr2: Byte;
  P1, P2: PChar;
begin
  Count1 := Length(S1);
  Count2 := Length(S2);
  if Count1 > Count2 then
    Count := Count2
  else
    Count := Count1;
  I := 0;
  if Count > 0 then
    begin
      P1 := @S1[1];
      P2 := @S2[1];
      while I < Count do
        begin
          Chr1 := Byte(P1^);
          Chr2 := Byte(P2^);
          if Chr1 <> Chr2 then
            begin
              if Chr1 in [97..122] then
                Dec(Chr1, 32);
              if Chr2 in [97..122] then
                Dec(Chr2, 32);
              if Chr1 <> Chr2 then
                Break
            end;
          Inc(P1);
          Inc(P2);
          Inc(I)
        end
    end;
  if I < Count then
    Result := Chr1 - Chr2
  else
    Result := Count1 - Count2
end;

procedure RunErrorToException(ErrorCode: LongInt;
  Address, Frame: Pointer);
var
  E: Exception;
begin
  case ErrorCode of
    201: E := ERangeError.Create('Range check error');
    215: E := EIntOverflow.Create('Arithmetic overflow');
    203: E := EOutOfMemory.Create('Out of memory');
    211: E := EAbstractError.Create('Abstract method called');
    216: E := EAccessViolation.Create('Access violation');
    2, 3, 4, 5, 6, 12, 15, 100, 101, 102, 103, 104, 105, 106, 156:
      begin
        E := EInOutError.Create('I/O error');
        EInOutError(E).ErrorCode := ErrorCode
      end
  else
    E := Exception.Create('Runtime error')
  end;
  raise E at Address, Frame
end;

procedure ReportUnhandledException(E: TObject;
  Address, Frame: Pointer);
begin
  if Assigned(E) and (Address = Frame) then
    WriteLn('Unhandled Pascal exception')
  else
    WriteLn('Unhandled Pascal exception')
end;

constructor Exception.Create(const Msg: String);
begin
  FMessage := Msg;
  FHelpContext := 0
end;

constructor Exception.CreateHelp(const Msg: String; AHelpContext: LongInt);
begin
  FMessage := Msg;
  FHelpContext := AHelpContext
end;

initialization
  ErrorProc := @RunErrorToException;
  ExceptProc := @ReportUnhandledException;

finalization
  ExceptProc := nil;
  ErrorProc := nil;

end.
