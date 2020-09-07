FROM mcr.microsoft.com/dotnet/core/aspnet:3.1-buster-slim AS base
WORKDIR /app
EXPOSE 80
EXPOSE 443

FROM mcr.microsoft.com/dotnet/core/sdk:3.1-buster AS build
WORKDIR /src
COPY Karenia.Rurikawa.Coordinator.csproj ./
RUN dotnet restore
COPY ./* .
RUN dotnet build -c Release -o /app/build -v m

FROM build AS publish
RUN dotnet publish -c Release -o /app/publish

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .
ENTRYPOINT ["dotnet", "Karenia.Rurikawa.Coordinator.dll"]
