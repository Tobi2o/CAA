# CAA projet

## Auteur

- **Nom** : Ouweis Harun

## Analyse des Fonctionnalités

1. **Authentification des utilisateurs** :
   - Les utilisateurs doivent pouvoir se connecter via un nom d'utilisateur et un mot de passe.
   - Le système doit sécuriser les informations d'identification à travers un mécanisme robuste de hachage et de salage des mots de passe.

2. **Gestion des comptes** :
   - Les utilisateurs doivent pouvoir changer leur mot de passe.
   - Une procédure de récupération de mot de passe en cas d'oubli devrait être envisagée, mais n'est pas obligatoire pour ce projet.

3. **Envoi et réception des messages** :
   - Les utilisateurs peuvent envoyer des messages contenant du texte ou des fichiers de grande taille.
   - Les messages doivent inclure une date de déverrouillage après laquelle ils peuvent être lus par le destinataire.
   - Les messages ne doivent être déchiffrables par le destinataire qu'à partir de la date spécifiée et doivent rester confidentiels jusqu'à cette date.

4. **Confidentialité et intégrité des messages** :
   - Implémenter le chiffrement de bout en bout pour garantir que seuls les utilisateurs concernés puissent lire les messages.
   - Assurer l'intégrité des messages pour que les données envoyées ne puissent pas être altérées sans être détectées.

5. **Scalabilité et performance** :
   - Le système doit être capable de gérer un grand nombre d'utilisateurs et de messages sans dégradation significative des performances.
   - Optimiser les ressources serveur, surtout en ce qui concerne les opérations de chiffrement et de déchiffrement.

### Définition de la Sécurité

- **Niveau de Sécurité** : Fixer à 256-bit pour aligner avec les standards actuels de sécurité.
- **Protocoles de Sécurité** : Utiliser des protocoles éprouvés et des algorithmes de chiffrement reconnus pour assurer la confidentialité et l'intégrité des données.

## Architecture Générale

### 1. **Authentification des Utilisateurs**

- **Fonctionnalité** : Authentification simple basée sur un nom d'utilisateur et un mot de passe.
- **Implémentation** :
  - **Stockage des mots de passe** :
    - Utiliser PBKDF2 avec HMAC-SHA256 pour dériver un hachage sécurisé du mot de passe.
    - Ajouter un sel unique de 128 bits à chaque utilisateur pour empêcher les attaques par table de hachage.
  - **Protocoles associés** :
    - Au moment de la création du compte, le mot de passe est haché (PBKDF2 + Sel) et le résultat est stocké dans la base de données.
    - Lors de la connexion, comparer le hachage du mot de passe fourni par l'utilisateur avec celui stocké.
  - **Détails de sécurité** :
    - Taille de la clé dérivée : 256 bits.
    - Itérations : 100 000 (augmentation pour renforcer la sécurité contre le matériel moderne).
    - Mécanisme anti-rejeu : Verrouiller le compte après plusieurs tentatives infructueuses.

---

### 2. **Envoi et Réception des Messages**

- **Fonctionnalité** : Les messages ne sont accessibles qu’après une date spécifique.
- **Implémentation** :
  - **Chiffrement Symétrique** :
    - Algorithme : AES-256-GCM.
    - Le message est chiffré avec une clé symétrique unique générée aléatoirement pour chaque message.
  - **Partage de la clé symétrique** :
    - Algorithme : ECC avec courbe secp256r1 (ou Curve25519 pour des performances accrues).
    - La clé symétrique est chiffrée avec la clé publique du destinataire.
  - **Gestion de la date** :
    - Inclure la date de déchiffrement dans le métadonné du message, signée par l’expéditeur pour garantir son authenticité.
- **Détails de sécurité** :
  - Taille de la clé AES : 256 bits.
  - IV pour AES-GCM : 96 bits, généré aléatoirement.
  - AuthTag (pour intégrité) : 128 bits.
  - Clés ECC : 256 bits.
  - **Optimisation pour les fichiers volumineux** :
    - Découper les fichiers en morceaux chiffrés individuellement.
    - Stocker une seule clé AES utilisée pour tous les morceaux.

---

### 3. **Confidentialité et Intégrité des Messages**

- **Fonctionnalité** : Empêcher la lecture ou l’altération par des tiers.
- **Implémentation** :
  - **Confidentialité** :
    - Le chiffrement avec AES-GCM garantit que seul le destinataire peut accéder au contenu.
  - **Intégrité** :
    - Le tag d’authentification (AuthTag) généré par AES-GCM assure que le contenu n’a pas été modifié.
- **Protocole de transmission sécurisé** :
  - Tous les messages transitent via TLS 1.3 entre le client et le serveur pour prévenir les attaques MITM.

---

### 4. **Gestion et Stockage des Clés**

- **Fonctionnalité** : Assurer une gestion sécurisée des clés dans un système avec des millions d’utilisateurs.
- **Implémentation** :
  - **Clés publiques/privées des utilisateurs** :
    - Génération locale sur le client via une bibliothèque cryptographique Rust (comme *ring* ou *dalek*).
    - Stockage local des clés privées avec chiffrement par un mot de passe maître dérivé via PBKDF2.
  - **Stockage des clés publiques** :
    - Les clés publiques sont stockées sur le serveur dans une base de données chiffrée.
    - Mise en cache pour éviter une surcharge du serveur.
  - **Rotation des clés** :
    - Les clés symétriques (AES) sont renouvelées pour chaque message.
    - Les clés publiques/privées ECC sont renouvelées annuellement ou en cas de compromission.

---

### 5. **Non-Répudiation**

- **Fonctionnalité** : Garantir que l’expéditeur ne peut pas nier l’envoi d’un message.
- **Implémentation** :
  - **Signature numérique** :
    - Algorithme : ECDSA avec la même courbe ECC (secp256r1 ou Curve25519).
    - L’expéditeur signe un hash du message avant de le chiffrer.
    - Le destinataire peut vérifier cette signature en utilisant la clé publique de l’expéditeur.
- **Détails de sécurité** :
  - Hash : SHA-256.

---

### 6. **Scalabilité et Performance**

- **Fonctionnalité** : Supporter des millions d’utilisateurs et de messages.
- **Implémentation** :
  - **Base de données sécurisée** :
    - Utiliser une base NoSQL distribuée (comme MongoDB avec chiffrement natif).
  - **Compression avant chiffrement** :
    - Compresser les messages ou fichiers avant le chiffrement pour réduire les tailles de transfert et de stockage.
  - **Caching** :
    - Mettre en cache les clés publiques fréquemment utilisées.

---

### 7. **Protocoles Associés**

- **Authentification mutuelle TLS** :
  - Utiliser des certificats clients pour renforcer la sécurité.
- **Protocole de récupération des clés publiques** :
  - Implémenter un mécanisme sécurisé pour permettre aux clients de récupérer ou de mettre à jour les clés publiques.
